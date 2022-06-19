<template>
    <div v-if="!retrievalError">
        <v-card
            elevation="0"
            justify="center"
            align="center"
            color="secondary"
            class="white--text"
        >   
            <div></div>
            <v-avatar size="300">
                <img src="../../assets/defaultCommunityIcon.jpg" />
            </v-avatar>
            <h1>
                <b> {{ communityInfo.title }} </b>
            </h1>
            <div v-if="subscribed">
                <v-btn color="secondary" @click="unsubscribe()">
                    Unsubscribe
                </v-btn>
            </div>
            <div v-else>
                <v-btn color="secondary" @click="subscribe()">
                    Subscribe
                </v-btn>
            </div>

            <v-divider></v-divider>
        </v-card>

        <v-tabs v-model="tab" color="secondary" right>
            <v-tab href="#posts">
                Posts
            </v-tab>
            <v-tab href="#about">
                About
            </v-tab>
        </v-tabs>

        <v-tabs-items :value="tab">
            <v-tab-item value="posts">
                <CommunityPosts />
            </v-tab-item>
            <v-tab-item value="about">
                <CommunityDescriptionCard :communityInfo="communityInfo" />
            </v-tab-item>
        </v-tabs-items>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </div>
    <div v-else>
        <p>{{ errorMessage }}</p>
        <v-btn color="primary" @click="getCommunityInfo">
            Try Again
        </v-btn>
    </div>
</template>

<script>
import CommunityPosts from './CommunityPosts.vue'
import CommunityDescriptionCard from './CommunityDescriptionCard.vue'

export default {
    mounted() {
        this.getCommunityInfo()
    },
    components: {
        CommunityPosts,
        CommunityDescriptionCard,
    },
    data() {
        return {
            subscribed: false,
            moderator: false,
            tabItems: ['posts', 'about', 'moderators'],
            tab: null,
            communityInfo: {
                id: this.$route.params.communityID,
                title: '',
                description: '',
                moderators: [],
                created: '',
            },
            currentUserId: localStorage.getItem('userid'),

            // error handling
            retrievalError: false,
            errorSnackbar: false,
            errorMessage: '',
        }
    },
    methods: {
        getCommunityInfo() {
            // try and get information about this community

            this.retrievalError = false
            const url = `/internal/communities/${this.communityInfo.id}`

            this.$http
                .get(url, {}, { withCredentials: true })
                .then(response => {
                    // data in response?
                    if (response.data.length !== 0) {
                        this.communityInfo = response.data
                    } else {
                        this.retrievalError = true
                        this.errorMessage =
                            'There was an error getting community data :('
                    }
                })
                .catch(error => {
                    this.retrievalError = true
                    this.errorMessage =
                        'There was an error contacting the server :('
                })

            // get user subscriptions, if this this page is in user subscriptions set subscribed = true
            const userUrl = `/internal/users/${this.currentUserId}`
            this.$http
                .get(userUrl, {}, { withCredentials: true })
                .then(response => {
                    if (response.data.length !== 0) {
                        const subscriptions = response.data.subscribed
                        for (let i = 0; i < subscriptions.length; i++) {
                            if (subscriptions[i] === this.communityInfo.id) {
                                this.subscribed = true
                            }
                        }
                    }
                })
                .catch(error => {
                    this.errorMessage = 'Error getting user subscription data'
                    this.errorSnackbar = true
                })
        },

        subscribe() {
            const url = `/internal/communities/${this.communityInfo.id}/subscribe`
            this.$http
                .post(url, {}, { withCredentials: true })
                .then(_ => {
                    this.subscribed = true
                })
                .catch(_ => {
                    this.errorMessage = 'Error when trying to subscribe'
                    this.errorSnackbar = true
                })
        },

        unsubscribe() {
            const url = `/internal/communities/${this.communityInfo.id}/subscribe`
            this.$http
                .delete(url, {}, { withCredentials: true })
                .then(_ => {
                    this.subscribed = false
                })
                .catch(_ => {
                    this.errorMessage = 'Error when trying to unsubscribe'
                    this.errorSnackbar = true
                })
        },
    },
}
</script>
