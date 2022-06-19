<template>
    <v-card elevation="0" color="secondary">
        <v-list two-line color="secondary">
            <v-divider></v-divider>
            <v-list-item>
                <v-list-item-avatar>
                    <v-img :src="userProfilePicLink" />
                </v-list-item-avatar>
                <v-list-item-content>
                    <v-list-item-title class="text-left white--text">{{
                        comment.content[0].text.text
                    }}</v-list-item-title>
                    <v-list-item-subtitle class="text-left white--text"
                        ><button @click="redirrectUserPage()">
                            {{ comment.author.username }}
                        </button>
                        • Posted at {{ getCreatedDate() }} •
                        <v-btn icon @click="overlay = !overlay">
                            <v-icon color="accent">
                                mdi-reply
                            </v-icon>
                        </v-btn>
                        <v-btn
                            icon
                            v-if="
                                checkPostBelongsToUser(comment.author.username)
                            "
                            @click="confirmDeleteDialog = !confirmDeleteDialog"
                        >
                            <v-icon color="red">mdi-delete</v-icon>
                        </v-btn>
                    </v-list-item-subtitle>
                </v-list-item-content>
            </v-list-item>
        </v-list>

        <v-overlay :z-index="zIndex" :value="overlay">
            <ReplyBox
                v-on:closeBox="overlay = false"
                v-on:postSuccesful="update"
                :item="comment"
                :id="comment.id"
            />
        </v-overlay>

        <div v-if="comment.children.length > 0">
            <v-row>
                <v-col :cols="1">
                    ↳
                </v-col>
                <v-col :cols="19">
                    <Comment
                        v-for="child in comment.children"
                        v-on:postSuccesful="update"
                        v-on:deleteComment="update"
                        :key="child.id"
                        :comment="child"
                    ></Comment>
                </v-col>
            </v-row>
        </div>
        <v-dialog v-model="confirmDeleteDialog" max-width="450">
            <v-card>
                <v-card-title
                    >Are you sure you want to delete this comment?</v-card-title
                >
                <v-card-actions>
                    <v-btn text plain @click="confirmDeleteDialog = false">
                        cancel
                    </v-btn>
                    <v-btn
                        text
                        color="red"
                        plain
                        @click="deleteComment(comment.id)"
                    >
                        delete
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
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
    </v-card>
</template>

<script>
import ReplyBox from './ReplyBox.vue'
export default {
    props: ['comment'],
    name: 'Comment',
    mounted() {
        this.getProfilePic()
    },
    data() {
        return {
            url: '/internal/posts',
            zIndex: 10,
            overlay: false,

            // confirm delete
            confirmDeleteDialog: false,

            // profile pic
            userProfilePicLink: require('../../assets/defaultUserIcon.jpg'),

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    components: {
        ReplyBox,
    },

    methods: {
        deleteComment(id) {
            const url = `${this.url}/${id}`

            this.$http
                .delete(url, {}, { withCredentials: true })
                .then(response => {
                    this.$emit('deleteComment', this.item)
                })
                .catch(error => {
                    this.errorMessage = 'Problem contacting the server'
                    this.errorSnackbar = true
                })
        },

        getProfilePic() {
            const url = `/internal/users/${this.comment.author.username}`

            this.$http
                .get(url, {}, {withCredentials: true})
                .then(response => {
                    if (response.data.length !== 0) {
                        const userInfo = response.data
                        if (userInfo.avatarUrl) {
                            this.userProfilePicLink = userInfo.avatarUrl
                        }
                    } 
                })
        },

        checkPostBelongsToUser(id) {
            return id === localStorage.getItem('userid')
        },

        update() {
            this.overlay = false
            this.$emit('postSuccesful')
        },

        getCreatedDate() {
            const d = new Date(0)
            d.setUTCSeconds(this.comment.created)
            return d.toUTCString()
        },

        redirrectUserPage() {
            const url = `/user/${this.comment.author.username}`
            this.$router.push(url)
        },
    },
}
</script>
