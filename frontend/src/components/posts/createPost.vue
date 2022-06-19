<template>
    <v-card width="1000px" color="secondary" class="createPostBox">
        <v-app-bar elevation="0" color="secondary">
            <v-toolbar-title>
                Create a post!
            </v-toolbar-title>

            <v-spacer> </v-spacer>

            <v-btn icon @click="close">
                <v-icon color="accent">
                    mdi-close
                </v-icon>
            </v-btn>
        </v-app-bar>
        <v-form ref="form" v-model="valid" lazy-validation max-width="80%">
            <div v-if="!previewMode">
                <v-list color="secondary">
                    <v-list-item>
                        <v-text-field
                            v-model="postTitle"
                            :rules="titleRules"
                            label="Title"
                            outlined
                        >
                        </v-text-field>
                    </v-list-item>
                    <v-list-item>
                        <v-textarea v-model="body" :rules="bodyRules" outlined>
                            <template v-slot:label>
                                <div>
                                    <b>body</b>
                                </div>
                            </template>
                        </v-textarea>
                    </v-list-item>
                </v-list>
                <v-card-text>
                    <p class="text-caption accent--text">Media</p>
                </v-card-text>
                <v-card-text>
                    <v-row class="px-4">
                        <div v-if="images.length <= 0">
                            <p>no media content</p>
                        </div>
                        <div v-else v-for="imageUrl in images" :key="imageUrl">
                            <v-img
                                :src="'/internal/images/' + imageUrl"
                                class="mx-2"
                                max-height="200px"
                                max-width="200px"
                            ></v-img>
                        </div>
                    </v-row>
                </v-card-text>
            </div>
            <div v-else>
                <v-card-text color="secondary">
                    <!-- render markdown of post -->
                    <VueMarkdown class="mx-2">{{ body }}</VueMarkdown>
                </v-card-text>
            </div>
            <v-card-actions color="secondary">
                <v-tooltip bottom>
                    <template v-slot:activator="{ on, attrs }">
                        <v-btn
                            icon
                            v-bind="attrs"
                            v-on="on"
                            @click="showPreview()"
                        >
                            <v-icon color="accent">mdi-eye</v-icon>
                        </v-btn>
                    </template>
                    <span>Toggle Preview</span>
                </v-tooltip>
                <!-- only accept images for now -->
                <v-file-input
                    accept="image/*"
                    class="mx-3"
                    hide-input
                    @change="uploadImage"
                    v-model="currentImage"
                >
                </v-file-input>

                <v-spacer></v-spacer>
                <v-btn icon :disabled="!valid" @click="handleSubmit" x-large>
                    <v-icon color="accent">mdi-send</v-icon>
                </v-btn>
            </v-card-actions>
        </v-form>
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
import VueMarkdown from 'vue-markdown'

export default {
    props: ['communityID'],
    components: {
        VueMarkdown,
    },
    data() {
        return {
            valid: true,
            postTitle: '',

            // preview components
            previewMode: false,

            // managing post body
            body: '',
            content: [],

            // rules to validate form.
            titleRules: [v => !!v || 'Title Required!'],
            bodyRules: [v => !!v || 'Body Required'],
            url: '/internal/posts',
            imageAPIUrl: '/internal/images',

            // image handling
            imageUrls: [],
            images: [],
            currentImage: null,

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    methods: {
        handleSubmit() {
            // validate form
            if (this.$refs.form.validate()) {
                console.log(this.body)
                // send post request with new form details
                this.$http
                    .post(
                        this.url,
                        {
                            community: this.communityID,
                            title: this.postTitle,
                            content: [
                                {
                                    markdown: {
                                        text: this.body,
                                    },
                                },
                            ],
                        },
                        { withCredentials: true }
                    )
                    .then(response => {
                        console.log('post was succesful')
                        // on success, emit postSuccessful, parent uses this to update list of posts
                        this.$emit('postSuccesful', 'success')
                        this.errorMessage = ''
                    })
                    .catch(error => {
                        this.errorMessage =
                            'There was a problem contacting the server'
                        this.errorSnackbar = true
                    })
            }
        },

        close() {
            this.$emit('close')
        },

        uploadImage() {
            const images = this.images.slice()

            // send to server
            const formData = new FormData()
            formData.append('file', this.currentImage)
            this.$http
                .post(this.imageAPIUrl, formData, {
                    headers: {
                        'Content-Type': 'multipart/form-data',
                    },
                    withCredentials: true,
                })
                .then(response => {
                    if (response.data) {
                        const id = response.data
                        // keep an array of the ids
                        images.push(id)
                        this.images = images

                        let currentBody = this.body

                        // string for image in markdown
                        // html version
                        const markdownImageText = `<img src="/internal/images/${id}" width="300" />`
                        // let markdownImageText = `![alt text](/internal/images/${id} "${this.currentImage.name}")`
                        currentBody +=
                            currentBody === ''
                                ? markdownImageText
                                : `\n${markdownImageText}`

                        this.body = currentBody
                    } else {
                        this.errorMessage = 'Error uploading photo'
                        this.errorSnackbar = true
                    }
                })
                .catch(err => {
                    this.errorMessage = 'Error uploading photo'
                    this.errorSnackbar = true
                })
        },

        showPreview() {
            this.previewMode = !this.previewMode
        },
    },
}
</script>

<style scoped>
.createPostBox {
    overflow: scroll;
    max-height: 1000px;
}
</style>
